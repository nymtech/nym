// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnet_contract_common::reward_params::RewardParams;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(export, export_to = "../../nym-wallet/src/types/rust/mixnodestatus.ts")
)]
#[serde(rename_all = "snake_case")]
pub enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set, but is bonded
    NotFound, // doesn't even exist in the bonded set
}

impl MixnodeStatus {
    pub fn is_active(&self) -> bool {
        *self == MixnodeStatus::Active
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(
        export,
        export_to = "../../nym-wallet/src/types/rust/corenodestatusresponse.ts"
    )
)]
pub struct CoreNodeStatusResponse {
    pub identity: String,
    pub count: i32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(
        export,
        export_to = "../../nym-wallet/src/types/rust/mixnodestatusresponse.ts"
    )
)]
pub struct MixnodeStatusResponse {
    pub status: MixnodeStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RewardEstimationResponse {
    pub estimated_total_node_reward: u64,
    pub estimated_operator_reward: u64,
    pub estimated_delegators_reward: u64,

    pub reward_params: RewardParams,
    pub as_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UptimeResponse {
    pub identity: String,
    pub avg_uptime: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(
        export,
        export_to = "../../nym-wallet/src/types/rust/stakesaturationresponse.ts"
    )
)]
pub struct StakeSaturationResponse {
    pub saturation: f32,
    pub as_at: i64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(
    test,
    ts(
        export,
        export_to = "../../nym-wallet/src/types/rust/inclusionprobabilityresponse.ts"
    )
)]
pub struct InclusionProbabilityResponse {
    pub in_active: f32,
    pub in_reserve: f32,
}

impl fmt::Display for InclusionProbabilityResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "in_active: {:.5}, in_reserve: {:.5}",
            self.in_active, self.in_reserve
        )
    }
}
