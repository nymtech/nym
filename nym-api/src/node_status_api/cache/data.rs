// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated};

use crate::support::caching::Cache;

use super::inclusion_probabilities::InclusionProbabilities;

#[derive(Default)]
pub(crate) struct NodeStatusCacheData {
    pub(crate) mixnodes_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    pub(crate) rewarded_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,
    pub(crate) active_set_annotated: Cache<Vec<MixNodeBondAnnotated>>,

    pub(crate) gateways_annotated: Cache<Vec<GatewayBondAnnotated>>,

    // Estimated active set inclusion probabilities from Monte Carlo simulation
    pub(crate) inclusion_probabilities: Cache<InclusionProbabilities>,
}

impl NodeStatusCacheData {
    pub fn new() -> Self {
        Self::default()
    }
}
