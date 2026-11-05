// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract_window::WindowData;
use crate::epoch_operations::EpochAdvancer;
use crate::epoch_operations::helpers::stake_to_f64;
use crate::sampling::sample_weighted_with_filler;
use anyhow::bail;
use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::{Performance, RewardedSetParams};
use nym_mixnet_contract_common::{
    EpochState, ExecuteMsg, NodeId, NymNodeDetails, RewardedSet, RewardingParams, RoleAssignment,
};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use rand::seq::SliceRandom;
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
struct NodeWithSaturationAndPerformance {
    node_id: NodeId,
    available_roles: Vec<AvailableRole>,
    saturation: Decimal,
    performance: Performance,
}

impl NodeWithSaturationAndPerformance {
    fn to_selection_weight(&self) -> f64 {
        let scaled_performance = match self.performance.checked_pow(20) {
            Ok(perf) => perf,
            Err(overflow) => {
                warn!(
                    "the node's performance ({}) has overflow while scaling it by the factor of 20: {overflow}. Setting it to 0 instead.",
                    self.performance
                );
                return 0.;
            }
        };

        let scaled_stake = self.saturation * scaled_performance;
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
        nodes: Vec<NodeWithSaturationAndPerformance>,
        spec: RewardedSetParams,
    ) -> anyhow::Result<RewardedSet> {
        if nodes.is_empty() {
            warn!("there are no nodes for assignment!");
            return Ok(RewardedSet::default());
        }

        let mut rng = rand::rng();

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
        let entry_gateways = sample_weighted_with_filler(
            &mut rng,
            &entry_eligible,
            spec.entry_gateways as usize,
            |item| item.1,
        )?
        .into_iter()
        .map(|node| node.0.node_id)
        .collect::<HashSet<_>>();

        // 2. determine exit gateways
        let exit_eligible = all_choices
            .iter()
            .filter(|node| {
                node.0.can_operate_exit_gateway() && !entry_gateways.contains(&node.0.node_id)
            })
            .collect::<Vec<_>>();
        let exit_gateways = sample_weighted_with_filler(
            &mut rng,
            &exit_eligible,
            spec.exit_gateways as usize,
            |item| item.1,
        )?
        .into_iter()
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
        let mixnodes =
            sample_weighted_with_filler(&mut rng, &mix_eligible, spec.mixnodes as usize, |item| {
                item.1
            })?
            .into_iter()
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
        let standby = sample_weighted_with_filler(
            &mut rng,
            &standby_eligible,
            spec.standby as usize,
            |item| item.1,
        )?
        .into_iter()
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

    fn attach_performance_to_eligible_nodes(
        &self,
        nym_nodes: &[NymNodeDetails],
        reward_params: &RewardingParams,
    ) -> Vec<NodeWithSaturationAndPerformance> {
        if !self.window.current_epoch_has_data() {
            warn!(
                "nothing was submitted to the performance contract for epoch {}: selecting the rewarded set by stake alone",
                self.window.current_epoch()
            );
        }
        eligible_nodes_with_performance(nym_nodes, reward_params, &self.window)
    }

    pub(super) async fn update_rewarded_set_and_advance_epoch(
        &self,
        nym_nodes: &[NymNodeDetails],
    ) -> anyhow::Result<()> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            EpochState::RoleAssignment { next } => {
                // with how the driver is currently coded, this should never happen as we're always
                // assigning roles to ALL nodes at once, but who knows what we might decide to do in the future...
                if !next.is_first() {
                    bail!(
                        "it seems the current epoch is in mid-role assignment state (next role to assign is {next}). With our current driver this shouldn't have been possible. Manual intervention is required."
                    )
                }

                info!("attempting to assign the rewarded set for the upcoming epoch...");

                if let Err(err) = self._update_rewarded_set_and_advance_epoch(nym_nodes).await {
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
                bail!(
                    "the current epoch is in the wrong state ({state}) to perform the requested operation: assigning rewarded set"
                )
            }
        }
    }

    async fn _update_rewarded_set_and_advance_epoch(
        &self,
        nym_nodes: &[NymNodeDetails],
    ) -> anyhow::Result<()> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_rewarding_parameters().await?;

        debug!("Rewarding parameters: {rewarding_parameters:?}");

        let nodes_with_performance =
            self.attach_performance_to_eligible_nodes(nym_nodes, &rewarding_parameters);

        let new_rewarded_set =
            self.determine_rewarded_set(nodes_with_performance, rewarding_parameters.rewarded_set)?;

        debug!("New rewarded set: {:?}", new_rewarded_set);

        self.send_role_assignment_messages(new_rewarded_set).await?;
        Ok(())
    }

    async fn send_role_assignment_messages(&self, rewarded_set: RewardedSet) -> anyhow::Result<()> {
        let mixnet_contract = self.mixnet_contract()?;
        let msgs = generate_role_assignment_messages(rewarded_set);

        self.nyxd_client
            .execute_multiple(
                &mixnet_contract,
                msgs,
                None,
                "assigning all the rewarded set roles",
            )
            .await?;
        Ok(())
    }
}

fn generate_role_assignment_messages(rewarded_set: RewardedSet) -> Vec<(ExecuteMsg, Vec<Coin>)> {
    // currently we just assign all of them together,
    // but the contract is ready to handle them separately should we need it
    // if the tx is too big
    let mut msgs = Vec::new();
    for (role, nodes) in [
        (Role::ExitGateway, rewarded_set.exit_gateways),
        (Role::EntryGateway, rewarded_set.entry_gateways),
        (Role::Layer1, rewarded_set.layer1),
        (Role::Layer2, rewarded_set.layer2),
        (Role::Layer3, rewarded_set.layer3),
        (Role::Standby, rewarded_set.standby),
    ] {
        msgs.push((
            ExecuteMsg::AssignRoles {
                assignment: RoleAssignment { role, nodes },
            },
            Vec::new(),
        ));
    }
    msgs
}

/// The candidates for the rewarded set, each with the performance it is selected by: nodes
/// declaring a role they can take, skipped without a performance.
fn eligible_nodes_with_performance(
    nym_nodes: &[NymNodeDetails],
    reward_params: &RewardingParams,
    window: &WindowData,
) -> Vec<NodeWithSaturationAndPerformance> {
    let mut with_performance = Vec::new();

    for nym_node in nym_nodes {
        let node_id = nym_node.node_id();
        let saturation = nym_node.rewarding_details.bond_saturation(reward_params);

        let Some(performance) = window.selection_performance(node_id) else {
            debug!("no performance for nym-node {node_id}");
            continue;
        };
        debug!("nym-node {node_id}: saturation: {saturation}, performance: {performance}");

        let available_roles: Vec<AvailableRole> =
            todo!("the node's roles come from its directory record");

        if available_roles.is_empty() {
            warn!("nym-node {node_id} can't operate under any mode!");
            continue;
        }

        with_performance.push(NodeWithSaturationAndPerformance {
            node_id,
            available_roles,
            saturation,
            performance,
        })
    }

    with_performance
}
