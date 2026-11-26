// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::contract_window::WindowData;
use crate::epoch_operations::EpochAdvancer;
use crate::epoch_operations::helpers::stake_to_f64;
use crate::network_view::{AvailableRole, NetworkView, SelectionCandidate};
use crate::sampling::sample_weighted_with_filler;
use anyhow::bail;
use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::{Performance, RewardedSetParams};
use nym_mixnet_contract_common::{
    EpochState, ExecuteMsg, NodeId, RewardedSet, RewardingParams, RoleAssignment,
};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::MixnetQueryClient;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use tracing::{debug, error, info, warn};

/// A candidate with the weight it is drawn by.
#[derive(Debug, Clone)]
struct WeightedCandidate<'a> {
    node_id: NodeId,
    roles: &'a [AvailableRole],
    weight: f64,
}

/// `saturation x performance^20`.
fn selection_weight(saturation: Decimal, performance: Performance) -> f64 {
    let scaled_performance = match performance.checked_pow(20) {
        Ok(perf) => perf,
        Err(overflow) => {
            warn!(
                "the node's performance ({performance}) has overflow while scaling it by the factor of 20: {overflow}. Setting it to 0 instead."
            );
            return 0.;
        }
    };
    stake_to_f64(saturation * scaled_performance)
}

/// Draws the entry gateways first, then the exit gateways, the mixnodes and the standby nodes,
/// each role from the candidates not drawn for an earlier one.
fn determine_rewarded_set(
    candidates: &[WeightedCandidate],
    spec: RewardedSetParams,
) -> anyhow::Result<RewardedSet> {
    if candidates.is_empty() {
        warn!("there are no nodes for assignment!");
        return Ok(RewardedSet::default());
    }

    let mut rng = rand::rng();
    let mut drawn = HashSet::new();

    // `None` for standby, which any role will do for
    let mut draw = |role: Option<AvailableRole>, count: u32, name: &str| {
        let eligible = candidates
            .iter()
            .filter(|candidate| {
                !drawn.contains(&candidate.node_id)
                    && role
                        .as_ref()
                        .is_none_or(|role| candidate.roles.contains(role))
            })
            .collect::<Vec<_>>();
        let mut nodes =
            sample_weighted_with_filler(&mut rng, &eligible, count as usize, |candidate| {
                candidate.weight
            })?
            .into_iter()
            .map(|candidate| candidate.node_id)
            .collect::<Vec<_>>();

        if nodes.len() != count as usize {
            warn!(
                "we didn't manage to select {count} {name}. we only got {}",
                nodes.len()
            )
        }
        drawn.extend(nodes.iter().copied());
        nodes.sort();
        anyhow::Ok(nodes)
    };

    let entry_gateways = draw(
        Some(AvailableRole::EntryGateway),
        spec.entry_gateways,
        "entry gateways",
    )?;
    let exit_gateways = draw(
        Some(AvailableRole::ExitGateway),
        spec.exit_gateways,
        "exit gateways",
    )?;
    let mut mixnodes = draw(Some(AvailableRole::Mix), spec.mixnodes, "mixnodes")?;
    let standby = draw(None, spec.standby, "standby nodes")?;

    // the layers: the drawn mixnodes shuffled and dealt out in turn
    mixnodes.shuffle(&mut rng);
    let mut layers: [Vec<NodeId>; 3] = Default::default();
    for (i, node_id) in mixnodes.into_iter().enumerate() {
        layers[i % 3].push(node_id);
    }
    for layer in &mut layers {
        layer.sort();
    }
    let [layer1, layer2, layer3] = layers;

    Ok(RewardedSet {
        entry_gateways,
        exit_gateways,
        layer1,
        layer2,
        layer3,
        standby,
    })
}

impl EpochAdvancer {
    pub(super) async fn update_rewarded_set_and_advance_epoch(
        &self,
        network_view: &NetworkView,
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

                if let Err(err) = self
                    ._update_rewarded_set_and_advance_epoch(network_view)
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
                bail!(
                    "the current epoch is in the wrong state ({state}) to perform the requested operation: assigning rewarded set"
                )
            }
        }
    }

    async fn _update_rewarded_set_and_advance_epoch(
        &self,
        network_view: &NetworkView,
    ) -> anyhow::Result<()> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_rewarding_parameters().await?;

        debug!("Rewarding parameters: {rewarding_parameters:?}");

        let candidates = weighted_candidates(
            &network_view.candidates,
            &rewarding_parameters,
            &self.window,
        );

        let new_rewarded_set =
            determine_rewarded_set(&candidates, rewarding_parameters.rewarded_set)?;

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

/// The candidates selection draws from, each weighted by its saturation and its selection
/// performance; a candidate without a performance is skipped.
fn weighted_candidates<'a>(
    candidates: &'a [SelectionCandidate],
    reward_params: &RewardingParams,
    window: &WindowData,
) -> Vec<WeightedCandidate<'a>> {
    if !window.current_epoch_has_data() {
        warn!(
            "nothing was submitted to the performance contract for epoch {}: selecting the rewarded set by stake alone",
            window.current_epoch()
        );
    }

    let mut weighted = Vec::new();
    for candidate in candidates {
        let node_id = candidate.node_id;
        let saturation = candidate.rewarding.bond_saturation(reward_params);

        let Some(performance) = window.selection_performance(node_id) else {
            debug!("no performance for nym-node {node_id}");
            continue;
        };
        debug!("nym-node {node_id}: saturation: {saturation}, performance: {performance}");

        weighted.push(WeightedCandidate {
            node_id,
            roles: &candidate.roles,
            weight: selection_weight(saturation, performance),
        })
    }
    weighted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_is_drawn_from_its_own_candidates_once() {
        use AvailableRole::{EntryGateway, ExitGateway, Mix};

        // 30 candidates: entry only, exit only, mix only, and every role
        let roles = [
            vec![EntryGateway],
            vec![ExitGateway],
            vec![Mix],
            vec![Mix, EntryGateway, ExitGateway],
        ];
        let candidates = (0..30)
            .map(|node_id| WeightedCandidate {
                node_id,
                roles: &roles[node_id as usize % roles.len()],
                weight: 1. + node_id as f64,
            })
            .collect::<Vec<_>>();
        let spec = RewardedSetParams {
            entry_gateways: 4,
            exit_gateways: 4,
            mixnodes: 7,
            standby: 5,
        };

        let set = determine_rewarded_set(&candidates, spec).unwrap();
        let has_role = |node_id: &NodeId, role| candidates[*node_id as usize].roles.contains(&role);

        assert_eq!(set.entry_gateways.len(), 4);
        assert_eq!(set.exit_gateways.len(), 4);
        assert_eq!(set.layer1.len() + set.layer2.len() + set.layer3.len(), 7);
        assert_eq!(set.standby.len(), 5);
        assert!(
            set.entry_gateways
                .iter()
                .all(|id| has_role(id, EntryGateway))
        );
        assert!(set.exit_gateways.iter().all(|id| has_role(id, ExitGateway)));

        // the layers differ by at most one node
        let layers = [&set.layer1, &set.layer2, &set.layer3];
        for layer in layers {
            assert!(layer.iter().all(|id| has_role(id, Mix)));
            assert!((2..=3).contains(&layer.len()));
        }

        // every list sorted, and no node in two of them
        let lists = [
            &set.entry_gateways,
            &set.exit_gateways,
            &set.layer1,
            &set.layer2,
            &set.layer3,
            &set.standby,
        ];
        assert!(lists.iter().all(|list| list.is_sorted()));
        let all = lists
            .iter()
            .flat_map(|list| list.iter())
            .collect::<Vec<_>>();
        assert_eq!(all.iter().collect::<HashSet<_>>().len(), all.len());
    }
}
