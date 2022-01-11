// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mix_nodes::models::MixNodeSummary;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct GatewaySummary {
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct ValidatorSummary {
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct OverviewSummary {
    pub mixnodes: MixNodeSummary,
    pub gateways: GatewaySummary,
    pub validators: ValidatorSummary,
}
