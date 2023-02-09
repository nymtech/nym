// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::stake_to_f64;
use crate::RewardedSetUpdater;
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{IdentityKey, Layer, LayerAssignment, MixNodeDetails};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashMap;

// Weight of a layer being chose is reciprocal to current count in layer
fn layer_weight(l: &Layer, layer_assignments: &HashMap<Layer, f32>) -> f32 {
    let total = layer_assignments.values().fold(0., |acc, i| acc + i);
    if total == 0. {
        1.
    } else {
        1. - (layer_assignments.get(l).unwrap_or(&0.) / total)
    }
}

impl RewardedSetUpdater {
    async fn determine_layers(
        &self,
        rewarded_set: &[MixNodeDetails],
    ) -> Result<(Vec<LayerAssignment>, HashMap<String, Layer>), RewardingError> {
        let mut families_in_layer: HashMap<String, Layer> = HashMap::new();
        let mut assignments = vec![];
        let mut layer_assignments: HashMap<Layer, f32> = HashMap::new();
        let mut rng = OsRng;
        let layers = vec![Layer::One, Layer::Two, Layer::Three];

        let mix_to_family = self.nym_contract_cache.mix_to_family().await.to_vec();

        let mix_to_family = mix_to_family
            .into_iter()
            .collect::<HashMap<IdentityKey, FamilyHead>>();

        for mix in rewarded_set {
            let family = mix_to_family.get(&mix.bond_information.identity().to_owned());
            // Get layer already assigned to nodes family, if any
            let family_layer = family.and_then(|h| families_in_layer.get(h.identity()));

            // Same node families are always assigned to the same layer, otherwise layer selected by a random weighted choice
            let layer = if let Some(layer) = family_layer {
                layer.to_owned()
            } else {
                layers
                    .choose_weighted(&mut rng, |l| layer_weight(l, &layer_assignments))?
                    .to_owned()
            };

            assignments.push(LayerAssignment::new(mix.mix_id(), layer));

            // layer accounting
            let layer_entry = layer_assignments.entry(layer).or_insert(0.);
            *layer_entry += 1.;
            if let Some(family) = family {
                families_in_layer.insert(family.identity().to_string(), layer);
            }
        }

        Ok((assignments, families_in_layer))
    }

    fn determine_rewarded_set(
        &self,
        mixnodes: &[MixNodeDetails],
        nodes_to_select: u32,
    ) -> Result<Vec<MixNodeDetails>, RewardingError> {
        if mixnodes.is_empty() {
            return Ok(Vec::new());
        }

        let mut rng = OsRng;

        // generate list of mixnodes and their relatively weight (by total stake)
        let choices = mixnodes
            .iter()
            .map(|mix| {
                let total_stake = stake_to_f64(mix.total_stake());
                (mix.to_owned(), total_stake)
            })
            .collect::<Vec<_>>();

        // the unwrap here is fine as an error can only be thrown under one of the following conditions:
        // - our mixnode list is empty - we have already checked for that
        // - we have invalid weights, i.e. less than zero or NaNs - it shouldn't happen in our case as we safely cast down from u128
        // - all weights are zero - it's impossible in our case as the list of nodes is not empty and weight is proportional to stake. You must have non-zero stake in order to bond
        // - we have more than u32::MAX values (which is incredibly unrealistic to have 4B mixnodes bonded... literally every other person on the planet would need one)
        Ok(choices
            .choose_multiple_weighted(&mut rng, nodes_to_select as usize, |item| item.1)?
            .map(|(mix, _weight)| mix.to_owned())
            .collect())
    }

    pub(super) async fn update_rewarded_set_and_advance_epoch(
        &self,
        all_mixnodes: &[MixNodeDetails],
    ) -> Result<(), RewardingError> {
        // we grab rewarding parameters here as they might have gotten updated when performing epoch actions
        let rewarding_parameters = self.nyxd_client.get_current_rewarding_parameters().await?;

        let new_rewarded_set =
            self.determine_rewarded_set(all_mixnodes, rewarding_parameters.rewarded_set_size)?;

        let (layer_assignments, _families_in_layer) =
            self.determine_layers(&new_rewarded_set).await?;

        self.nyxd_client
            .advance_current_epoch(layer_assignments, rewarding_parameters.active_set_size)
            .await?;

        Ok(())
    }
}
