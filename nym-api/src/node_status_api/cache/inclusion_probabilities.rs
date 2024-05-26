// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::error;
use nym_api_requests::models::InclusionProbability;
use nym_contracts_common::truncate_decimal;
use nym_mixnet_contract_common::{MixId, MixNodeDetails, RewardingParams};
use serde::Serialize;
use std::time::Duration;
use tap::TapFallible;

const MAX_SIMULATION_SAMPLES: u64 = 5000;
const MAX_SIMULATION_TIME_SEC: u64 = 15;

#[derive(Clone, Default, Serialize, schemars::JsonSchema)]
pub(crate) struct InclusionProbabilities {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
}

impl InclusionProbabilities {
    pub(crate) fn compute(
        mixnodes: &[MixNodeDetails],
        params: RewardingParams,
    ) -> Option<InclusionProbabilities> {
        compute_inclusion_probabilities(mixnodes, params)
    }

    pub(crate) fn node(&self, mix_id: MixId) -> Option<&InclusionProbability> {
        self.inclusion_probabilities
            .iter()
            .find(|x| x.mix_id == mix_id)
    }
}

fn compute_inclusion_probabilities(
    mixnodes: &[MixNodeDetails],
    params: RewardingParams,
) -> Option<InclusionProbabilities> {
    let active_set_size = params.active_set_size;
    let standby_set_size = params.rewarded_set_size - active_set_size;

    // Unzip list of total bonds into ids and bonds.
    // We need to go through this zip/unzip procedure to make sure we have matching identities
    // for the input to the simulator, which assumes the identity is the position in the vec
    let (ids, mixnode_total_bonds) = unzip_into_mixnode_ids_and_total_bonds(mixnodes);

    // Compute inclusion probabilitites and keep track of how long time it took.
    let mut rng = rand::thread_rng();
    let results = nym_inclusion_probability::simulate_selection_probability_mixnodes(
        &mixnode_total_bonds,
        active_set_size as usize,
        standby_set_size as usize,
        MAX_SIMULATION_SAMPLES,
        Duration::from_secs(MAX_SIMULATION_TIME_SEC),
        &mut rng,
    )
    .tap_err(|err| error!("{err}"))
    .ok()?;

    Some(InclusionProbabilities {
        inclusion_probabilities: zip_ids_together_with_results(&ids, &results),
        samples: results.samples,
        elapsed: results.time,
        delta_max: results.delta_max,
        delta_l2: results.delta_l2,
    })
}

fn unzip_into_mixnode_ids_and_total_bonds(mixnodes: &[MixNodeDetails]) -> (Vec<MixId>, Vec<u128>) {
    mixnodes
        .iter()
        .map(|m| (m.mix_id(), truncate_decimal(m.total_stake()).u128()))
        .unzip()
}

fn zip_ids_together_with_results(
    ids: &[MixId],
    results: &nym_inclusion_probability::SelectionProbability,
) -> Vec<InclusionProbability> {
    ids.iter()
        .zip(results.active_set_probability.iter())
        .zip(results.reserve_set_probability.iter())
        .map(|((&mix_id, a), r)| InclusionProbability {
            mix_id,
            in_active: *a,
            in_reserve: *r,
        })
        .collect()
}
