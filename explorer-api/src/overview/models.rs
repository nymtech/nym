// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

use crate::gateways::models::GatewaySummary;
use crate::mix_nodes::models::MixNodeSummary;
use crate::validators::models::ValidatorSummary;

#[derive(Clone, Debug, Serialize, JsonSchema, Default)]
pub(crate) struct RoleSummary {
    pub mixnode: usize,
    pub entry: usize,
    pub exit_nr: usize,
    pub exit_ipr: usize,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Default)]
pub(crate) struct NymNodeSummary {
    pub count: usize,
    pub roles: RoleSummary,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct OverviewSummary {
    pub mixnodes: MixNodeSummary,
    pub gateways: GatewaySummary,
    pub validators: ValidatorSummary,
    pub nymnodes: NymNodeSummary,
}
