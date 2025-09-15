// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(deprecated)]

use nym_api_requests::legacy::LegacyMixNodeDetailsWithLayer;
use nym_api_requests::models::InclusionProbability;
use nym_mixnet_contract_common::NodeId;
use serde::Serialize;
use std::time::Duration;

#[deprecated]
#[derive(Clone, Default, Serialize, schemars::JsonSchema)]
pub(crate) struct InclusionProbabilities {
    pub inclusion_probabilities: Vec<InclusionProbability>,
    pub samples: u64,
    pub elapsed: Duration,
    pub delta_max: f64,
    pub delta_l2: f64,
}

impl InclusionProbabilities {
    pub(crate) fn legacy_zero(
        mixnodes: &[LegacyMixNodeDetailsWithLayer],
    ) -> InclusionProbabilities {
        // (all legacy mixnodes have 0% chance of being selected)
        InclusionProbabilities {
            inclusion_probabilities: mixnodes
                .iter()
                .map(|m| InclusionProbability {
                    mix_id: m.mix_id(),
                    in_active: 0.0,
                    in_reserve: 0.0,
                })
                .collect(),
            samples: 0,
            elapsed: Default::default(),
            delta_max: 0.0,
            delta_l2: 0.0,
        }
    }

    pub(crate) fn node(&self, mix_id: NodeId) -> Option<&InclusionProbability> {
        self.inclusion_probabilities
            .iter()
            .find(|x| x.mix_id == mix_id)
    }
}
