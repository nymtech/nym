// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::stake_to_f64;
use crate::RewardedSetUpdater;
use cosmwasm_std::Decimal;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{
    EpochState, IdentityKey, Interval, Layer, LayerAssignment, MixId, MixNodeDetails,
};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct MixnodeWithStakeAndPerformance {
    mix_id: MixId,
    identity: IdentityKey,
    total_stake: Decimal,
    performance: Performance,
}

impl MixnodeWithStakeAndPerformance {
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
}

impl RewardedSetUpdater {
    // Needs to run for active and reserve sets separatley, as it does not preserve order
    async fn determine_layers(
        &self,
        set: &[MixnodeWithStakeAndPerformance],
    ) -> Result<Vec<LayerAssignment>, RewardingError> {
        let mut assignments = Vec::with_capacity(set.len());
        let target_layer_count = set.len() / 3;

        let mix_to_family = self.nym_contract_cache.mix_to_family().await.to_vec();

        let mix_to_family = mix_to_family
            .into_iter()
            .collect::<HashMap<IdentityKey, FamilyHead>>();

        let mut regular_nodes = Vec::with_capacity(set.len());

        let mut families = HashMap::new();

        for node in set.iter() {
            if let Some(fh) = mix_to_family.get(&node.identity) {
                let family: &mut Vec<u32> = families.entry(fh.identity()).or_default();
                family.push(node.mix_id)
            } else {
                regular_nodes.push(node.mix_id)
            }
        }

        let mut layers = HashMap::new();
        layers.insert(Layer::One, Vec::with_capacity(target_layer_count));
        layers.insert(Layer::Two, Vec::with_capacity(target_layer_count));
        layers.insert(Layer::Three, Vec::with_capacity(target_layer_count));

        // Assign all members of a family to same layer
        for (_head, members) in families.iter_mut() {
            let smallest_layer = layers
                .iter()
                .min_by_key(|(_layer, members)| members.len())
                .map(|(layer, _members)| *layer)
                .unwrap_or(Layer::One);

            let entry = layers.entry(smallest_layer).or_default();
            if entry.len() + members.len() <= target_layer_count {
                entry.extend_from_slice(members)
            }
        }

        // Assign nodes with no families into layers
        for mix_id in regular_nodes.drain(..) {
            let smallest_layer = layers
                .iter()
                .min_by_key(|(_layer, members)| members.len())
                .map(|(layer, _members)| *layer)
                .unwrap_or(Layer::One);

            let entry = layers.entry(smallest_layer).or_default();
            if entry.len() < target_layer_count {
                entry.push(mix_id)
            }
        }

        for (layer, members) in layers {
            let layer_assignments = members
                .into_iter()
                .map(|mix_id| LayerAssignment::new(mix_id, layer));
            assignments.extend(layer_assignments);
        }
        Ok(assignments)
    }

    fn determine_rewarded_set(
        &self,
        mixnodes: Vec<MixnodeWithStakeAndPerformance>,
        nodes_to_select: u32,
    ) -> Result<Vec<MixnodeWithStakeAndPerformance>, RewardingError> {
        if mixnodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .into_iter()
            .map(|mix| {
                let weight = mix.to_selection_weight();
                (mix, weight)
            })
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        Ok(choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)?
            .map(|(mix, _weight)| mix.clone())
            .collect())
    }

    async fn attach_performance(
        &self,
        interval: Interval,
        mixnodes: &[MixNodeDetails],
    ) -> Vec<MixnodeWithStakeAndPerformance> {
        let mut with_performance = Vec::with_capacity(mixnodes.len());
        for mix in mixnodes {
            with_performance.push(MixnodeWithStakeAndPerformance {
                mix_id: mix.mix_id(),
                identity: mix.bond_information.identity().to_owned(),
                total_stake: mix.total_stake(),
                performance: self
                    .load_performance(&interval, mix.mix_id())
                    .await
                    .performance,
            })
        }
        with_performance
    }

    pub(super) async fn update_rewarded_set_and_advance_epoch(
        &self,
        current_interval: Interval,
        all_mixnodes: &[MixNodeDetails],
    ) -> Result<(), RewardingError> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            EpochState::AdvancingEpoch => {
                log::info!("Advancing epoch and updating the rewarded set...");
                let nodes_with_performance = self
                    .attach_performance(current_interval, all_mixnodes)
                    .await;

                if let Err(err) = self
                    ._update_rewarded_set_and_advance_epoch(nodes_with_performance)
                    .await
                {
                    log::error!("FAILED to advance the current epoch... - {err}");
                    Err(err)
                } else {
                    log::info!("Advanced the epoch and updated the rewarded set... SUCCESS");
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
        all_mixnodes: Vec<MixnodeWithStakeAndPerformance>,
    ) -> Result<(), RewardingError> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_current_rewarding_parameters().await?;

        debug!("Rewarding paremeters: {:?}", rewarding_parameters);

        let new_rewarded_set =
            self.determine_rewarded_set(all_mixnodes, rewarding_parameters.rewarded_set_size)?;

        debug!("New rewarded set: {:?}", new_rewarded_set);

        let empty = vec![];

        let (active_set, reserve_set) = if new_rewarded_set.len()
            <= rewarding_parameters.active_set_size as usize
        {
            warn!("Active set size ({}) is greater then rewarded set len ({}), there will be no reserve set", rewarding_parameters.active_set_size, new_rewarded_set.len());
            (new_rewarded_set.as_slice(), empty.as_slice())
        } else {
            new_rewarded_set.split_at(rewarding_parameters.active_set_size as usize)
        };

        let mut active_set_layer_assignments = self.determine_layers(active_set).await?;
        debug!(
            "Active set layer assignments: {:?}",
            active_set_layer_assignments
        );
        let reserve_set_layer_assignments = self.determine_layers(reserve_set).await?;
        debug!(
            "Reserve set layer assignments: {:?}",
            reserve_set_layer_assignments
        );

        active_set_layer_assignments.extend(reserve_set_layer_assignments);

        debug!(
            "Rewarded set layer assignments: {:?}",
            active_set_layer_assignments
        );

        self.nyxd_client
            .advance_current_epoch(
                active_set_layer_assignments,
                rewarding_parameters.active_set_size,
            )
            .await?;

        Ok(())
    }
}
